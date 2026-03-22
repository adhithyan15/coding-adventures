# frozen_string_literal: true

# ==========================================================================
# Tests for the Starlark AST-to-Bytecode Compiler
# ==========================================================================
#
# These tests verify that Starlark source code is correctly compiled into
# bytecode instructions. Each test compiles a source string and checks
# the resulting CodeObject for the expected instructions, constants, and names.
#
# Tests are organized by language feature, progressing from simple to complex:
#   1. Basic assignment and arithmetic
#   2. Comparison and boolean operators
#   3. Control flow (if/else, for)
#   4. Functions
#   5. Collections (list, dict, tuple)
#   6. Advanced features (load, lambda, etc.)
# ==========================================================================

require "minitest/autorun"
require "coding_adventures_starlark_ast_to_bytecode_compiler"

class TestCompiler < Minitest::Test
  Compiler = CodingAdventures::StarlarkAstToBytecodeCompiler::Compiler
  Op = CodingAdventures::StarlarkAstToBytecodeCompiler::Op

  # ================================================================
  # Test Helpers
  # ================================================================

  # Compile source code and return the CodeObject.
  def compile(source)
    Compiler.compile_starlark(source)
  end

  # Check that the instruction at the given index has the expected opcode.
  def assert_opcode(code, index, expected_opcode)
    assert index < code.instructions.length,
      "Instruction index #{index} out of range (only #{code.instructions.length} instructions)\n" \
      "Disassembly:\n#{Compiler.disassemble(code)}"
    actual = code.instructions[index].opcode
    assert_equal expected_opcode, actual,
      "instruction[#{index}]: expected #{Op::NAMES[expected_opcode]} (0x#{expected_opcode.to_s(16)}), " \
      "got #{Op::NAMES[actual] || "UNKNOWN"} (0x#{actual.to_s(16)})\n" \
      "Disassembly:\n#{Compiler.disassemble(code)}"
  end

  # Check that the instruction at the given index has the expected operand.
  def assert_operand(code, index, expected_operand)
    assert index < code.instructions.length,
      "Instruction index #{index} out of range"
    assert_equal expected_operand, code.instructions[index].operand,
      "instruction[#{index}] operand: expected #{expected_operand.inspect}, " \
      "got #{code.instructions[index].operand.inspect}\n" \
      "Disassembly:\n#{Compiler.disassemble(code)}"
  end

  # Check if any instruction in the code has the given opcode.
  def has_opcode?(code, opcode)
    code.instructions.any? { |instr| instr.opcode == opcode }
  end

  # ================================================================
  # Basic Assignment Tests
  # ================================================================

  def test_simple_assignment
    # x = 42
    # Expected: LOAD_CONST 0, STORE_NAME 0, HALT
    code = compile("x = 42\n")

    assert_opcode(code, 0, Op::LOAD_CONST)
    assert_operand(code, 0, 0)
    assert_opcode(code, 1, Op::STORE_NAME)
    assert_operand(code, 1, 0)
    assert_opcode(code, 2, Op::HALT)

    assert_equal 42, code.constants[0]
    assert_equal "x", code.names[0]
  end

  def test_string_assignment
    # name = "hello"
    code = compile("name = \"hello\"\n")

    assert_opcode(code, 0, Op::LOAD_CONST)
    assert_opcode(code, 1, Op::STORE_NAME)
    assert_opcode(code, 2, Op::HALT)

    assert_equal "hello", code.constants[0]
    assert_equal "name", code.names[0]
  end

  def test_multiple_assignments
    # x = 1
    # y = 2
    code = compile("x = 1\ny = 2\n")

    assert_opcode(code, 0, Op::LOAD_CONST)
    assert_opcode(code, 1, Op::STORE_NAME)
    assert_opcode(code, 2, Op::LOAD_CONST)
    assert_opcode(code, 3, Op::STORE_NAME)
    assert_opcode(code, 4, Op::HALT)

    assert_equal 1, code.constants[0]
    assert_equal 2, code.constants[1]
    assert_equal "x", code.names[0]
    assert_equal "y", code.names[1]
  end

  # ================================================================
  # Arithmetic Tests
  # ================================================================

  def test_addition
    # x = 1 + 2
    code = compile("x = 1 + 2\n")

    assert_opcode(code, 0, Op::LOAD_CONST)
    assert_opcode(code, 1, Op::LOAD_CONST)
    assert_opcode(code, 2, Op::ADD)
    assert_opcode(code, 3, Op::STORE_NAME)
    assert_opcode(code, 4, Op::HALT)

    assert_equal 1, code.constants[0]
    assert_equal 2, code.constants[1]
  end

  def test_subtraction
    code = compile("x = 10 - 3\n")
    assert_opcode(code, 0, Op::LOAD_CONST)
    assert_opcode(code, 1, Op::LOAD_CONST)
    assert_opcode(code, 2, Op::SUB)
  end

  def test_multiplication
    code = compile("x = 3 * 4\n")
    assert_opcode(code, 0, Op::LOAD_CONST)
    assert_opcode(code, 1, Op::LOAD_CONST)
    assert_opcode(code, 2, Op::MUL)
  end

  def test_division
    code = compile("x = 10 / 3\n")
    assert_opcode(code, 0, Op::LOAD_CONST)
    assert_opcode(code, 1, Op::LOAD_CONST)
    assert_opcode(code, 2, Op::DIV)
  end

  def test_floor_division
    code = compile("x = 7 // 2\n")
    assert_opcode(code, 0, Op::LOAD_CONST)
    assert_opcode(code, 1, Op::LOAD_CONST)
    assert_opcode(code, 2, Op::FLOOR_DIV)
  end

  def test_modulo
    code = compile("x = 7 % 3\n")
    assert_opcode(code, 0, Op::LOAD_CONST)
    assert_opcode(code, 1, Op::LOAD_CONST)
    assert_opcode(code, 2, Op::MOD)
  end

  def test_exponentiation
    code = compile("x = 2 ** 10\n")
    assert_opcode(code, 0, Op::LOAD_CONST)
    assert_opcode(code, 1, Op::LOAD_CONST)
    assert_opcode(code, 2, Op::POWER)
  end

  def test_chained_arithmetic
    # x = 1 + 2 + 3
    code = compile("x = 1 + 2 + 3\n")

    assert_opcode(code, 0, Op::LOAD_CONST) # 1
    assert_opcode(code, 1, Op::LOAD_CONST) # 2
    assert_opcode(code, 2, Op::ADD)        # 1 + 2
    assert_opcode(code, 3, Op::LOAD_CONST) # 3
    assert_opcode(code, 4, Op::ADD)        # (1+2) + 3
    assert_opcode(code, 5, Op::STORE_NAME) # x = ...
  end

  # ================================================================
  # Unary Operator Tests
  # ================================================================

  def test_unary_negation
    code = compile("x = -5\n")
    assert has_opcode?(code, Op::NEGATE),
      "Expected NEGATE opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_bitwise_not
    code = compile("x = ~0\n")
    assert has_opcode?(code, Op::BIT_NOT),
      "Expected BIT_NOT opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_logical_not
    code = compile("x = not True\n")
    assert has_opcode?(code, Op::NOT),
      "Expected NOT opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  # ================================================================
  # Boolean Literal Tests
  # ================================================================

  def test_boolean_true
    code = compile("x = True\n")
    assert_opcode(code, 0, Op::LOAD_TRUE)
    assert_opcode(code, 1, Op::STORE_NAME)
  end

  def test_boolean_false
    code = compile("x = False\n")
    assert_opcode(code, 0, Op::LOAD_FALSE)
    assert_opcode(code, 1, Op::STORE_NAME)
  end

  def test_none
    code = compile("x = None\n")
    assert_opcode(code, 0, Op::LOAD_NONE)
    assert_opcode(code, 1, Op::STORE_NAME)
  end

  # ================================================================
  # Comparison Tests
  # ================================================================

  def test_comparison_equal
    code = compile("x = 1 == 2\n")
    assert has_opcode?(code, Op::CMP_EQ),
      "Expected CMP_EQ opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_comparison_not_equal
    code = compile("x = 1 != 2\n")
    assert has_opcode?(code, Op::CMP_NE),
      "Expected CMP_NE opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_comparison_less_than
    code = compile("x = 1 < 2\n")
    assert has_opcode?(code, Op::CMP_LT),
      "Expected CMP_LT opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_comparison_greater_than
    code = compile("x = 1 > 2\n")
    assert has_opcode?(code, Op::CMP_GT),
      "Expected CMP_GT opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_comparison_less_equal
    code = compile("x = 1 <= 2\n")
    assert has_opcode?(code, Op::CMP_LE),
      "Expected CMP_LE opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_comparison_greater_equal
    code = compile("x = 1 >= 2\n")
    assert has_opcode?(code, Op::CMP_GE),
      "Expected CMP_GE opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_comparison_in
    code = compile("x = 1 in [1, 2]\n")
    assert has_opcode?(code, Op::CMP_IN),
      "Expected CMP_IN opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_comparison_not_in
    code = compile("x = 3 not in [1, 2]\n")
    assert has_opcode?(code, Op::CMP_NOT_IN),
      "Expected CMP_NOT_IN opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  # ================================================================
  # Boolean Operator Tests (Short-Circuit)
  # ================================================================

  def test_boolean_or
    code = compile("x = a or b\n")
    assert has_opcode?(code, Op::JUMP_IF_TRUE_OR_POP),
      "Expected JUMP_IF_TRUE_OR_POP opcode for 'or'\n" \
      "Disassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_boolean_and
    code = compile("x = a and b\n")
    assert has_opcode?(code, Op::JUMP_IF_FALSE_OR_POP),
      "Expected JUMP_IF_FALSE_OR_POP opcode for 'and'\n" \
      "Disassembly:\n#{Compiler.disassemble(code)}"
  end

  # ================================================================
  # If/Else Tests
  # ================================================================

  def test_if_statement
    source = "if True:\n    x = 1\n"
    code = compile(source)

    assert_opcode(code, 0, Op::LOAD_TRUE)
    assert_opcode(code, 1, Op::JUMP_IF_FALSE)

    assert has_opcode?(code, Op::STORE_NAME),
      "Expected STORE_NAME in if body\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_if_else_statement
    source = "if True:\n    x = 1\nelse:\n    x = 2\n"
    code = compile(source)

    assert has_opcode?(code, Op::JUMP_IF_FALSE),
      "Expected JUMP_IF_FALSE opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
    assert has_opcode?(code, Op::JUMP),
      "Expected JUMP opcode for else branch\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  # ================================================================
  # For Loop Tests
  # ================================================================

  def test_for_loop
    source = "for x in [1, 2, 3]:\n    pass\n"
    code = compile(source)

    assert has_opcode?(code, Op::GET_ITER),
      "Expected GET_ITER opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
    assert has_opcode?(code, Op::FOR_ITER),
      "Expected FOR_ITER opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
    assert has_opcode?(code, Op::JUMP),
      "Expected JUMP opcode for loop back\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  # ================================================================
  # Function Definition Tests
  # ================================================================

  def test_simple_function
    source = "def f():\n    return 1\n"
    code = compile(source)

    assert has_opcode?(code, Op::MAKE_FUNCTION),
      "Expected MAKE_FUNCTION opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
    assert has_opcode?(code, Op::STORE_NAME),
      "Expected STORE_NAME opcode\nDisassembly:\n#{Compiler.disassemble(code)}"

    assert_equal "f", code.names[0]
  end

  def test_function_with_params
    source = "def add(a, b):\n    return a + b\n"
    code = compile(source)

    assert has_opcode?(code, Op::MAKE_FUNCTION),
      "Expected MAKE_FUNCTION opcode\nDisassembly:\n#{Compiler.disassemble(code)}"

    # Check that the nested CodeObject exists in constants
    assert code.constants.length > 0, "Expected at least one constant (the function info)"

    func_info = code.constants[0]
    assert_kind_of Hash, func_info

    params = func_info["params"]
    assert_equal %w[a b], params

    body_code = func_info["code"]
    assert_kind_of CodingAdventures::VirtualMachine::CodeObject, body_code

    # The nested CodeObject should contain ADD and RETURN_VALUE
    assert body_code.instructions.any? { |i| i.opcode == Op::ADD },
      "Expected ADD in function body"
    assert body_code.instructions.any? { |i| i.opcode == Op::RETURN_VALUE },
      "Expected RETURN_VALUE in function body"
  end

  # ================================================================
  # Function Call Tests
  # ================================================================

  def test_function_call_no_args
    code = compile("f()\n")

    found = code.instructions.find { |i| i.opcode == Op::CALL_FUNCTION }
    assert found, "Expected CALL_FUNCTION opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
    assert_equal 0, found.operand, "Expected 0 args"
  end

  def test_function_call_with_args
    code = compile("f(1, 2)\n")

    found = code.instructions.find { |i| i.opcode == Op::CALL_FUNCTION }
    assert found, "Expected CALL_FUNCTION opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
    assert_equal 2, found.operand, "Expected 2 args"
  end

  def test_function_call_with_kwargs
    code = compile("f(x=1, y=2)\n")

    assert has_opcode?(code, Op::CALL_FUNCTION_KW),
      "Expected CALL_FUNCTION_KW opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  # ================================================================
  # Collection Tests
  # ================================================================

  def test_empty_list
    code = compile("x = []\n")

    found = code.instructions.find { |i| i.opcode == Op::BUILD_LIST }
    assert found, "Expected BUILD_LIST opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
    assert_equal 0, found.operand, "Expected BUILD_LIST 0"
  end

  def test_list_literal
    code = compile("x = [1, 2, 3]\n")

    found = code.instructions.find { |i| i.opcode == Op::BUILD_LIST }
    assert found, "Expected BUILD_LIST opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
    assert_equal 3, found.operand, "Expected BUILD_LIST 3"
  end

  def test_empty_dict
    code = compile("x = {}\n")

    found = code.instructions.find { |i| i.opcode == Op::BUILD_DICT }
    assert found, "Expected BUILD_DICT opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
    assert_equal 0, found.operand, "Expected BUILD_DICT 0"
  end

  def test_dict_literal
    code = compile("x = {\"a\": 1, \"b\": 2}\n")

    found = code.instructions.find { |i| i.opcode == Op::BUILD_DICT }
    assert found, "Expected BUILD_DICT opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
    assert_equal 2, found.operand, "Expected BUILD_DICT 2"
  end

  def test_empty_tuple
    code = compile("x = ()\n")

    found = code.instructions.find { |i| i.opcode == Op::BUILD_TUPLE }
    assert found, "Expected BUILD_TUPLE opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
    assert_equal 0, found.operand, "Expected BUILD_TUPLE 0"
  end

  def test_tuple_literal
    code = compile("x = (1, 2)\n")

    assert has_opcode?(code, Op::BUILD_TUPLE),
      "Expected BUILD_TUPLE opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  # ================================================================
  # Attribute Access and Subscript Tests
  # ================================================================

  def test_attribute_access
    code = compile("x = obj.attr\n")

    assert has_opcode?(code, Op::LOAD_ATTR),
      "Expected LOAD_ATTR opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_subscript
    code = compile("x = lst[0]\n")

    assert has_opcode?(code, Op::LOAD_SUBSCRIPT),
      "Expected LOAD_SUBSCRIPT opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  # ================================================================
  # Load Statement Tests
  # ================================================================

  def test_load_statement
    code = compile("load(\"module.star\", \"symbol\")\n")

    assert has_opcode?(code, Op::LOAD_MODULE),
      "Expected LOAD_MODULE opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
    assert has_opcode?(code, Op::IMPORT_FROM),
      "Expected IMPORT_FROM opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  # ================================================================
  # Pass, Break, Continue Tests
  # ================================================================

  def test_pass_statement
    code = compile("pass\n")
    # pass is a no-op, so the only instruction should be HALT
    assert_opcode(code, 0, Op::HALT)
  end

  def test_break_statement
    source = "for x in [1]:\n    break\n"
    code = compile(source)

    assert has_opcode?(code, Op::BREAK),
      "Expected BREAK opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_continue_statement
    source = "for x in [1]:\n    continue\n"
    code = compile(source)

    assert has_opcode?(code, Op::CONTINUE),
      "Expected CONTINUE opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  # ================================================================
  # Return Statement Tests
  # ================================================================

  def test_return_none
    source = "def f():\n    return\n"
    code = compile(source)

    func_info = code.constants[0]
    body_code = func_info["code"]

    # First return should be LOAD_NONE + RETURN_VALUE
    assert_equal Op::LOAD_NONE, body_code.instructions[0].opcode
    assert_equal Op::RETURN_VALUE, body_code.instructions[1].opcode
  end

  def test_return_value
    source = "def f():\n    return 42\n"
    code = compile(source)

    func_info = code.constants[0]
    body_code = func_info["code"]

    assert_equal Op::LOAD_CONST, body_code.instructions[0].opcode
    assert_equal Op::RETURN_VALUE, body_code.instructions[1].opcode
  end

  # ================================================================
  # Bitwise Operator Tests
  # ================================================================

  def test_bitwise_and
    code = compile("x = 5 & 3\n")
    assert has_opcode?(code, Op::BIT_AND),
      "Expected BIT_AND opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_bitwise_or
    code = compile("x = 5 | 3\n")
    assert has_opcode?(code, Op::BIT_OR),
      "Expected BIT_OR opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_bitwise_xor
    code = compile("x = 5 ^ 3\n")
    assert has_opcode?(code, Op::BIT_XOR),
      "Expected BIT_XOR opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_left_shift
    code = compile("x = 1 << 3\n")
    assert has_opcode?(code, Op::LSHIFT),
      "Expected LSHIFT opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  def test_right_shift
    code = compile("x = 8 >> 2\n")
    assert has_opcode?(code, Op::RSHIFT),
      "Expected RSHIFT opcode\nDisassembly:\n#{Compiler.disassemble(code)}"
  end

  # ================================================================
  # Expression Statement Tests
  # ================================================================

  def test_expression_statement
    # A bare expression should be compiled and then popped
    code = compile("42\n")

    assert_opcode(code, 0, Op::LOAD_CONST)
    assert_opcode(code, 1, Op::POP)
    assert_opcode(code, 2, Op::HALT)
  end

  # ================================================================
  # Variable Reference Tests
  # ================================================================

  def test_variable_reference
    code = compile("x = 1\ny = x\n")

    # x = 1: LOAD_CONST 0, STORE_NAME 0
    assert_opcode(code, 0, Op::LOAD_CONST)
    assert_opcode(code, 1, Op::STORE_NAME)
    # y = x: LOAD_NAME 0, STORE_NAME 1
    assert_opcode(code, 2, Op::LOAD_NAME)
    assert_operand(code, 2, 0) # x is names[0]
    assert_opcode(code, 3, Op::STORE_NAME)
    assert_operand(code, 3, 1) # y is names[1]
  end

  # ================================================================
  # End-to-End Compilation Test
  # ================================================================

  def test_compile_starlark_end_to_end
    source = "x = 1 + 2\n"
    code = Compiler.compile_starlark(source)

    assert code.instructions.length >= 4,
      "Expected at least 4 instructions, got #{code.instructions.length}\n" \
      "Disassembly:\n#{Compiler.disassemble(code)}"

    assert_opcode(code, 0, Op::LOAD_CONST)
    assert_opcode(code, 1, Op::LOAD_CONST)
    assert_opcode(code, 2, Op::ADD)
    assert_opcode(code, 3, Op::STORE_NAME)
    assert_opcode(code, 4, Op::HALT)

    assert_equal 1, code.constants[0]
    assert_equal 2, code.constants[1]
    assert_equal "x", code.names[0]
  end

  def test_compile_ast
    ast = CodingAdventures::StarlarkParser.parse("x = 1\n")
    code = Compiler.compile_ast(ast)
    assert code.instructions.length >= 3,
      "Expected at least 3 instructions, got #{code.instructions.length}"
  end

  # ================================================================
  # Disassembly Test
  # ================================================================

  def test_disassemble
    code = compile("x = 42\n")
    output = Compiler.disassemble(code)

    refute_empty output
    assert_includes output, "LOAD_CONST"
    assert_includes output, "STORE_NAME"
    assert_includes output, "HALT"
  end

  # ================================================================
  # String Literal Parsing Tests
  # ================================================================

  def test_parse_string_literal_double_quoted
    assert_equal "hello", Compiler.parse_string_literal('"hello"')
  end

  def test_parse_string_literal_single_quoted
    assert_equal "world", Compiler.parse_string_literal("'world'")
  end

  def test_parse_string_literal_escape_newline
    assert_equal "a\nb", Compiler.parse_string_literal('"a\\nb"')
  end

  def test_parse_string_literal_escape_backslash
    assert_equal "a\\b", Compiler.parse_string_literal('"a\\\\b"')
  end

  def test_parse_string_literal_escape_quote
    assert_equal 'a"b', Compiler.parse_string_literal('"a\\"b"')
  end

  def test_parse_string_literal_raw
    assert_equal 'a\\nb', Compiler.parse_string_literal('r"a\\nb"')
  end

  def test_parse_string_literal_bare
    # Already stripped by lexer
    assert_equal "hello", Compiler.parse_string_literal("hello")
  end

  def test_parse_string_literal_empty
    assert_equal "", Compiler.parse_string_literal("")
  end

  # ================================================================
  # Operator Maps Test
  # ================================================================

  def test_binary_op_map_completeness
    expected_ops = %w[+ - * / // % ** << >> & | ^]
    expected_ops.each do |op|
      assert Compiler::BINARY_OP_MAP.key?(op),
        "BINARY_OP_MAP missing operator '#{op}'"
    end
  end

  def test_compare_op_map_completeness
    expected_ops = ["==", "!=", "<", ">", "<=", ">=", "in", "not in"]
    expected_ops.each do |op|
      assert Compiler::COMPARE_OP_MAP.key?(op),
        "COMPARE_OP_MAP missing operator '#{op}'"
    end
  end

  def test_augmented_assign_op_map_completeness
    expected_ops = %w[+= -= *= /= //= %= **= <<= >>= &= |= ^=]
    expected_ops.each do |op|
      assert Compiler::AUGMENTED_ASSIGN_OP_MAP.key?(op),
        "AUGMENTED_ASSIGN_OP_MAP missing operator '#{op}'"
    end
  end
end
