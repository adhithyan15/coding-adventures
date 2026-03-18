# frozen_string_literal: true

require_relative "test_helper"

# Comprehensive tests for the Bytecode Compiler.
#
# These tests verify that the compiler correctly translates AST nodes into
# bytecode instructions. We test at two levels:
#
# 1. Unit tests -- Feed hand-built AST nodes into the compiler and verify
#    the exact instructions, constants, and names that come out.
#
# 2. End-to-end tests -- Use compile_source to go from source code all the
#    way to a CodeObject, then execute it on the VM and check the results.

# Shorthand aliases
OC = CodingAdventures::VirtualMachine::OpCode
P = CodingAdventures::Parser

class TestNumberLiteral < Minitest::Test
  def test_number_literal_produces_load_const_pop_halt
    program = P::Program.new(statements: [P::NumberLiteral.new(value: 42)])
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)

    opcodes = code.instructions.map(&:opcode)
    assert_equal [OC::LOAD_CONST, OC::POP, OC::HALT], opcodes
    assert_equal [0, nil, nil], code.instructions.map(&:operand)
    assert_equal [42], code.constants
    assert_equal [], code.names
  end

  def test_number_literal_zero
    program = P::Program.new(statements: [P::NumberLiteral.new(value: 0)])
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)

    assert_equal [0], code.constants
    opcodes = code.instructions.map(&:opcode)
    assert_equal [OC::LOAD_CONST, OC::POP, OC::HALT], opcodes
  end
end

class TestStringLiteral < Minitest::Test
  def test_string_literal_produces_load_const_pop_halt
    program = P::Program.new(statements: [P::StringLiteral.new(value: "hello")])
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)

    opcodes = code.instructions.map(&:opcode)
    assert_equal [OC::LOAD_CONST, OC::POP, OC::HALT], opcodes
    assert_equal ["hello"], code.constants
  end

  def test_empty_string
    program = P::Program.new(statements: [P::StringLiteral.new(value: "")])
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)
    assert_equal [""], code.constants
  end
end

class TestNameReference < Minitest::Test
  def test_name_produces_load_name_pop_halt
    program = P::Program.new(statements: [P::Name.new(name: "x")])
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)

    opcodes = code.instructions.map(&:opcode)
    assert_equal [OC::LOAD_NAME, OC::POP, OC::HALT], opcodes
    assert_equal [], code.constants
    assert_equal ["x"], code.names
  end
end

class TestAssignment < Minitest::Test
  def test_simple_assignment
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 42)
        )
      ]
    )
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)

    opcodes = code.instructions.map(&:opcode)
    assert_equal [OC::LOAD_CONST, OC::STORE_NAME, OC::HALT], opcodes
    assert_equal [42], code.constants
    assert_equal ["x"], code.names
  end

  def test_assignment_with_string
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "name"),
          value: P::StringLiteral.new(value: "alice")
        )
      ]
    )
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)

    assert_equal ["alice"], code.constants
    assert_equal ["name"], code.names
  end
end

class TestBinaryOp < Minitest::Test
  def test_addition
    program = P::Program.new(
      statements: [
        P::BinaryOp.new(
          left: P::NumberLiteral.new(value: 1),
          op: "+",
          right: P::NumberLiteral.new(value: 2)
        )
      ]
    )
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)

    opcodes = code.instructions.map(&:opcode)
    assert_equal [OC::LOAD_CONST, OC::LOAD_CONST, OC::ADD, OC::POP, OC::HALT], opcodes
    assert_equal [1, 2], code.constants
  end

  def test_subtraction
    program = P::Program.new(
      statements: [
        P::BinaryOp.new(
          left: P::NumberLiteral.new(value: 5), op: "-",
          right: P::NumberLiteral.new(value: 3)
        )
      ]
    )
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)
    assert_includes code.instructions.map(&:opcode), OC::SUB
  end

  def test_multiplication
    program = P::Program.new(
      statements: [
        P::BinaryOp.new(
          left: P::NumberLiteral.new(value: 4), op: "*",
          right: P::NumberLiteral.new(value: 7)
        )
      ]
    )
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)
    assert_includes code.instructions.map(&:opcode), OC::MUL
  end

  def test_division
    program = P::Program.new(
      statements: [
        P::BinaryOp.new(
          left: P::NumberLiteral.new(value: 10), op: "/",
          right: P::NumberLiteral.new(value: 2)
        )
      ]
    )
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)
    assert_includes code.instructions.map(&:opcode), OC::DIV
  end
end

class TestComplexExpressions < Minitest::Test
  def test_assignment_with_binary_op
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::BinaryOp.new(
            left: P::NumberLiteral.new(value: 1), op: "+",
            right: P::NumberLiteral.new(value: 2)
          )
        )
      ]
    )
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)

    opcodes = code.instructions.map(&:opcode)
    assert_equal [OC::LOAD_CONST, OC::LOAD_CONST, OC::ADD, OC::STORE_NAME, OC::HALT], opcodes
    assert_equal [1, 2], code.constants
    assert_equal ["x"], code.names
  end

  def test_nested_binary_ops_respects_tree_structure
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::BinaryOp.new(
            left: P::NumberLiteral.new(value: 1), op: "+",
            right: P::BinaryOp.new(
              left: P::NumberLiteral.new(value: 2), op: "*",
              right: P::NumberLiteral.new(value: 3)
            )
          )
        )
      ]
    )
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)

    opcodes = code.instructions.map(&:opcode)
    assert_equal [OC::LOAD_CONST, OC::LOAD_CONST, OC::LOAD_CONST, OC::MUL, OC::ADD, OC::STORE_NAME, OC::HALT], opcodes
    assert_equal [1, 2, 3], code.constants
  end

  def test_binary_op_with_name_operands
    program = P::Program.new(
      statements: [
        P::BinaryOp.new(
          left: P::Name.new(name: "a"), op: "+",
          right: P::Name.new(name: "b")
        )
      ]
    )
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)

    opcodes = code.instructions.map(&:opcode)
    assert_equal [OC::LOAD_NAME, OC::LOAD_NAME, OC::ADD, OC::POP, OC::HALT], opcodes
    assert_equal ["a", "b"], code.names
  end
end

class TestMultipleStatements < Minitest::Test
  def test_two_assignments
    program = P::Program.new(
      statements: [
        P::Assignment.new(target: P::Name.new(name: "x"), value: P::NumberLiteral.new(value: 1)),
        P::Assignment.new(target: P::Name.new(name: "y"), value: P::NumberLiteral.new(value: 2))
      ]
    )
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)

    opcodes = code.instructions.map(&:opcode)
    assert_equal [OC::LOAD_CONST, OC::STORE_NAME, OC::LOAD_CONST, OC::STORE_NAME, OC::HALT], opcodes
    assert_equal [1, 2], code.constants
    assert_equal ["x", "y"], code.names
  end
end

class TestDeduplication < Minitest::Test
  def test_constant_deduplication
    program = P::Program.new(
      statements: [
        P::Assignment.new(target: P::Name.new(name: "x"), value: P::NumberLiteral.new(value: 1)),
        P::Assignment.new(target: P::Name.new(name: "y"), value: P::NumberLiteral.new(value: 1))
      ]
    )
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)

    assert_equal [1], code.constants
    load_consts = code.instructions.select { |i| i.opcode == OC::LOAD_CONST }
    assert(load_consts.all? { |i| i.operand == 0 })
  end

  def test_name_deduplication
    program = P::Program.new(
      statements: [
        P::Assignment.new(target: P::Name.new(name: "x"), value: P::NumberLiteral.new(value: 1)),
        P::Assignment.new(target: P::Name.new(name: "x"), value: P::NumberLiteral.new(value: 2))
      ]
    )
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)
    assert_equal ["x"], code.names
  end
end

class TestEmptyProgram < Minitest::Test
  def test_empty_program_produces_just_halt
    program = P::Program.new(statements: [])
    code = CodingAdventures::BytecodeCompiler::Compiler.new.compile(program)

    opcodes = code.instructions.map(&:opcode)
    assert_equal [OC::HALT], opcodes
    assert_equal [], code.constants
    assert_equal [], code.names
  end
end

class TestUnknownExpression < Minitest::Test
  def test_unknown_expression_raises_type_error
    compiler = CodingAdventures::BytecodeCompiler::Compiler.new
    assert_raises(TypeError) { compiler.compile_expression("not_an_ast_node") }
  end
end

class TestEndToEnd < Minitest::Test
  def test_simple_assignment
    code = CodingAdventures::BytecodeCompiler.compile_source("x = 1 + 2")
    vm = CodingAdventures::VirtualMachine::VM.new
    vm.execute(code)
    assert_equal 3, vm.variables["x"]
  end

  def test_multiple_assignments
    code = CodingAdventures::BytecodeCompiler.compile_source("a = 10\nb = 20\nc = a + b")
    vm = CodingAdventures::VirtualMachine::VM.new
    vm.execute(code)
    assert_equal 10, vm.variables["a"]
    assert_equal 20, vm.variables["b"]
    assert_equal 30, vm.variables["c"]
  end

  def test_arithmetic_operations
    code = CodingAdventures::BytecodeCompiler.compile_source(
      "a = 10 + 5\nb = 10 - 5\nc = 10 * 5\nd = 10 / 5"
    )
    vm = CodingAdventures::VirtualMachine::VM.new
    vm.execute(code)
    assert_equal 15, vm.variables["a"]
    assert_equal 5, vm.variables["b"]
    assert_equal 50, vm.variables["c"]
    assert_equal 2, vm.variables["d"]
  end

  def test_expression_with_precedence
    code = CodingAdventures::BytecodeCompiler.compile_source("x = 2 + 3 * 4")
    vm = CodingAdventures::VirtualMachine::VM.new
    vm.execute(code)
    assert_equal 14, vm.variables["x"]
  end

  def test_variable_reuse
    code = CodingAdventures::BytecodeCompiler.compile_source("x = 10\ny = x + 5")
    vm = CodingAdventures::VirtualMachine::VM.new
    vm.execute(code)
    assert_equal 10, vm.variables["x"]
    assert_equal 15, vm.variables["y"]
  end

  def test_compile_source_with_keywords
    code = CodingAdventures::BytecodeCompiler.compile_source("x = 1", keywords: ["if", "else"])
    vm = CodingAdventures::VirtualMachine::VM.new
    vm.execute(code)
    assert_equal 1, vm.variables["x"]
  end
end
