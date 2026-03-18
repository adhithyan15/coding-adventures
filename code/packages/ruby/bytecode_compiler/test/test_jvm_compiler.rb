# frozen_string_literal: true

require_relative "test_helper"

P = CodingAdventures::Parser unless defined?(P)

# Comprehensive tests for the JVM Bytecode Compiler.
#
# Tests verify the tiered encoding (iconst, bipush, ldc), local variable
# short/long forms, arithmetic opcodes, and end-to-end bytecode sequences.

class TestJVMNumberEncoding < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def compile_assignment(value)
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: value)
        )
      ]
    )
    JVMCompiler.new.compile(program)
  end

  def test_iconst_0
    code = compile_assignment(0)
    assert_equal ICONST_0, code.bytecode.getbyte(0)
  end

  def test_iconst_1
    code = compile_assignment(1)
    assert_equal ICONST_1, code.bytecode.getbyte(0)
  end

  def test_iconst_5
    code = compile_assignment(5)
    assert_equal ICONST_5, code.bytecode.getbyte(0)
  end

  def test_bipush_for_6
    code = compile_assignment(6)
    assert_equal BIPUSH, code.bytecode.getbyte(0)
    assert_equal 6, code.bytecode.getbyte(1)
  end

  def test_bipush_for_100
    code = compile_assignment(100)
    assert_equal BIPUSH, code.bytecode.getbyte(0)
    assert_equal 100, code.bytecode.getbyte(1)
  end

  def test_bipush_for_127
    code = compile_assignment(127)
    assert_equal BIPUSH, code.bytecode.getbyte(0)
    assert_equal 127, code.bytecode.getbyte(1)
  end

  def test_bipush_for_negative
    code = compile_assignment(-1)
    assert_equal BIPUSH, code.bytecode.getbyte(0)
    assert_equal 0xFF, code.bytecode.getbyte(1)
  end

  def test_ldc_for_128
    code = compile_assignment(128)
    assert_equal LDC, code.bytecode.getbyte(0)
    assert_equal 0, code.bytecode.getbyte(1) # constant pool index
    assert_equal [128], code.constants
  end

  def test_ldc_for_large_number
    code = compile_assignment(1000)
    assert_equal LDC, code.bytecode.getbyte(0)
    assert_includes code.constants, 1000
  end

  def test_ldc_for_negative_129
    code = compile_assignment(-129)
    assert_equal LDC, code.bytecode.getbyte(0)
    assert_includes code.constants, -129
  end
end

class TestJVMLocalVariableEncoding < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def test_istore_0
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 1)
        )
      ]
    )
    code = JVMCompiler.new.compile(program)
    # iconst_1 (1 byte), istore_0 (1 byte), return
    assert_equal ISTORE_0, code.bytecode.getbyte(1)
  end

  def test_istore_1
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 1)
        ),
        P::Assignment.new(
          target: P::Name.new(name: "y"),
          value: P::NumberLiteral.new(value: 2)
        )
      ]
    )
    code = JVMCompiler.new.compile(program)
    assert_equal ISTORE_1, code.bytecode.getbyte(3)
  end

  def test_istore_3
    program = P::Program.new(
      statements: [
        P::Assignment.new(target: P::Name.new(name: "a"), value: P::NumberLiteral.new(value: 0)),
        P::Assignment.new(target: P::Name.new(name: "b"), value: P::NumberLiteral.new(value: 1)),
        P::Assignment.new(target: P::Name.new(name: "c"), value: P::NumberLiteral.new(value: 2)),
        P::Assignment.new(target: P::Name.new(name: "d"), value: P::NumberLiteral.new(value: 3))
      ]
    )
    code = JVMCompiler.new.compile(program)
    assert_equal ISTORE_3, code.bytecode.getbyte(7)
  end

  def test_istore_generic_for_slot_4
    program = P::Program.new(
      statements: [
        P::Assignment.new(target: P::Name.new(name: "a"), value: P::NumberLiteral.new(value: 0)),
        P::Assignment.new(target: P::Name.new(name: "b"), value: P::NumberLiteral.new(value: 1)),
        P::Assignment.new(target: P::Name.new(name: "c"), value: P::NumberLiteral.new(value: 2)),
        P::Assignment.new(target: P::Name.new(name: "d"), value: P::NumberLiteral.new(value: 3)),
        P::Assignment.new(target: P::Name.new(name: "e"), value: P::NumberLiteral.new(value: 4))
      ]
    )
    code = JVMCompiler.new.compile(program)
    assert_equal ISTORE, code.bytecode.getbyte(9)
    assert_equal 4, code.bytecode.getbyte(10)
  end

  def test_iload_0
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 1)
        ),
        P::Assignment.new(
          target: P::Name.new(name: "y"),
          value: P::Name.new(name: "x")
        )
      ]
    )
    code = JVMCompiler.new.compile(program)
    assert_equal ILOAD_0, code.bytecode.getbyte(2)
  end

  def test_iload_generic_for_slot_4
    program = P::Program.new(
      statements: [
        P::Assignment.new(target: P::Name.new(name: "a"), value: P::NumberLiteral.new(value: 0)),
        P::Assignment.new(target: P::Name.new(name: "b"), value: P::NumberLiteral.new(value: 1)),
        P::Assignment.new(target: P::Name.new(name: "c"), value: P::NumberLiteral.new(value: 2)),
        P::Assignment.new(target: P::Name.new(name: "d"), value: P::NumberLiteral.new(value: 3)),
        P::Assignment.new(target: P::Name.new(name: "e"), value: P::NumberLiteral.new(value: 4)),
        P::Assignment.new(target: P::Name.new(name: "f"), value: P::Name.new(name: "e"))
      ]
    )
    code = JVMCompiler.new.compile(program)
    assert_equal ILOAD, code.bytecode.getbyte(11)
    assert_equal 4, code.bytecode.getbyte(12)
  end
end

class TestJVMArithmeticOps < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def compile_binop(op, left_val, right_val)
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::BinaryOp.new(
            left: P::NumberLiteral.new(value: left_val),
            op: op,
            right: P::NumberLiteral.new(value: right_val)
          )
        )
      ]
    )
    JVMCompiler.new.compile(program)
  end

  def test_iadd
    code = compile_binop("+", 1, 2)
    assert_includes code.bytecode.bytes, IADD
  end

  def test_isub
    code = compile_binop("-", 5, 3)
    assert_includes code.bytecode.bytes, ISUB
  end

  def test_imul
    code = compile_binop("*", 4, 3)
    assert_includes code.bytecode.bytes, IMUL
  end

  def test_idiv
    code = compile_binop("/", 10, 2)
    assert_includes code.bytecode.bytes, IDIV
  end
end

class TestJVMEndToEnd < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def test_x_equals_1_plus_2
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::BinaryOp.new(
            left: P::NumberLiteral.new(value: 1),
            op: "+",
            right: P::NumberLiteral.new(value: 2)
          )
        )
      ]
    )
    code = JVMCompiler.new.compile(program)

    expected = [ICONST_1, ICONST_2, IADD, ISTORE_0, JVM_RETURN].pack("C*").b
    assert_equal expected, code.bytecode
  end

  def test_empty_program
    program = P::Program.new(statements: [])
    code = JVMCompiler.new.compile(program)

    assert_equal [JVM_RETURN].pack("C*").b, code.bytecode
    assert_equal [], code.constants
    assert_equal 0, code.num_locals
    assert_equal [], code.local_names
  end

  def test_ends_with_return
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 42)
        )
      ]
    )
    code = JVMCompiler.new.compile(program)
    assert_equal JVM_RETURN, code.bytecode.getbyte(code.bytecode.bytesize - 1)
  end

  def test_expression_statement_emits_pop
    program = P::Program.new(
      statements: [
        P::BinaryOp.new(
          left: P::NumberLiteral.new(value: 1),
          op: "+",
          right: P::NumberLiteral.new(value: 2)
        )
      ]
    )
    code = JVMCompiler.new.compile(program)

    expected = [ICONST_1, ICONST_2, IADD, JVM_POP, JVM_RETURN].pack("C*").b
    assert_equal expected, code.bytecode
  end
end

class TestJVMConstantPool < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def test_constant_deduplication
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 200)
        ),
        P::Assignment.new(
          target: P::Name.new(name: "y"),
          value: P::NumberLiteral.new(value: 200)
        )
      ]
    )
    code = JVMCompiler.new.compile(program)
    assert_equal [200], code.constants
  end

  def test_string_in_constant_pool
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::StringLiteral.new(value: "hello")
        )
      ]
    )
    code = JVMCompiler.new.compile(program)
    assert_includes code.constants, "hello"
    assert_equal LDC, code.bytecode.getbyte(0)
  end
end

class TestJVMLocalNames < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def test_single_variable
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 1)
        )
      ]
    )
    code = JVMCompiler.new.compile(program)
    assert_equal ["x"], code.local_names
    assert_equal 1, code.num_locals
  end

  def test_reassignment_reuses_slot
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 1)
        ),
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 2)
        )
      ]
    )
    code = JVMCompiler.new.compile(program)
    assert_equal ["x"], code.local_names
    assert_equal 1, code.num_locals
  end
end

class TestJVMReturnType < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def test_returns_jvm_code_object
    program = P::Program.new(
      statements: [P::NumberLiteral.new(value: 1)]
    )
    code = JVMCompiler.new.compile(program)
    assert_instance_of JVMCodeObject, code
  end

  def test_bytecode_is_string
    program = P::Program.new(
      statements: [P::NumberLiteral.new(value: 1)]
    )
    code = JVMCompiler.new.compile(program)
    assert_instance_of String, code.bytecode
    assert_predicate code.bytecode, :frozen?
  end
end

class TestJVMErrorHandling < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def test_unknown_expression_raises_type_error
    compiler = JVMCompiler.new
    assert_raises(TypeError) { compiler.compile_expression("not_an_ast_node") }
  end
end
